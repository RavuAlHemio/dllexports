use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, Read};

use tracing::debug;

use crate::io_util::BitReader;


#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum HuffmanConstructionError {
    EmptySequence,
    PrefixFound { needle: Vec<bool>, haystack: Vec<bool> },
    SomeBranchesUndefined,
}
impl fmt::Display for HuffmanConstructionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySequence
                => write!(f, "mapping contains empty sequence"),
            Self::PrefixFound { needle, haystack }
                => write!(f, "sequence {:?} is a prefix of sequence {:?}", needle, haystack),
            Self::SomeBranchesUndefined
                => write!(f, "some branches are not defined"),
        }
    }
}
impl std::error::Error for HuffmanConstructionError {
}


/// A tree used for Huffman decoding.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct HuffmanTree<T> {
    root_node: BranchNode<T>,
}
impl<T> HuffmanTree<T> {
    /// Creates a new tree from the given mapping of bit sequences to symbols.
    ///
    /// The following criteria must be fulfilled:
    /// 1. Every sequence contains at least one element.
    /// 2. No sequence is a prefix of another sequence.
    /// 3. For every branch node, both children must be defined. For example, if the sequence 1001
    ///    is mapped to a symbol, sequences equal to or starting with 0, 11, 101 and 1000 must also
    ///    be defined.
    pub fn new(sequence_to_symbol: BTreeMap<Vec<bool>, T>) -> Result<Self, HuffmanConstructionError> {
        // protect against invalid Huffman trees:
        // 1. all sequences must be at least one element long
        for (sequence, _symbol) in &sequence_to_symbol {
            if sequence.len() < 1 {
                return Err(HuffmanConstructionError::EmptySequence);
            }
        }
        // 2. no sequence may be a prefix of another sequence
        for (needle, _symbol) in &sequence_to_symbol {
            for (haystack, _symbol) in &sequence_to_symbol {
                if haystack.starts_with(needle) && needle != haystack {
                    return Err(HuffmanConstructionError::PrefixFound {
                        needle: needle.clone(),
                        haystack: haystack.clone(),
                    });
                }
            }
        }

        // start constructing
        let mut root_node_uc = BranchNodeUnderConstruction::new();
        for (sequence, symbol) in sequence_to_symbol {
            let mut current_node_uc = &mut root_node_uc;

            // establish branches
            for &element in &sequence[..sequence.len() - 1] {
                let wanted_branch = if element {
                    &mut current_node_uc.true_child
                } else {
                    &mut current_node_uc.false_child
                };

                if wanted_branch.is_none() {
                    *wanted_branch = Some(Box::new(HuffmanNodeUnderConstruction::Branch(BranchNodeUnderConstruction::new())));
                }
                current_node_uc = match wanted_branch.as_mut().unwrap().as_branch_mut() {
                    Some(wb) => wb,
                    None => unreachable!("sequence-not-a-prefix-of-sequence invariant violated"),
                };
            }

            // hang up the final node
            let last_element = *sequence.last().unwrap();
            let wanted_leaf = if last_element {
                &mut current_node_uc.true_child
            } else {
                &mut current_node_uc.false_child
            };
            if wanted_leaf.is_some() {
                unreachable!("reached an existing node through sequence {:?}", sequence);
            }
            *wanted_leaf = Some(Box::new(HuffmanNodeUnderConstruction::Leaf(LeafNode::new(symbol))));
        }

        if !root_node_uc.is_finished() {
            return Err(HuffmanConstructionError::SomeBranchesUndefined);
        }
        let root_node: BranchNode<T> = match root_node_uc.try_into() {
            Ok(rn) => rn,
            Err(_) => unreachable!(),
        };

        Ok(Self {
            root_node,
        })
    }

    pub fn decode_one<I: Iterator<Item = bool>>(&self, mut iterator: I) -> Option<&T> {
        let mut current_node = &self.root_node;
        loop {
            // find out which branch to take
            let take_true_branch = iterator.next()?;
            let branch_taken = if take_true_branch {
                &*current_node.true_child
            } else {
                &*current_node.false_child
            };
            match branch_taken {
                HuffmanNode::Branch(b) => {
                    current_node = b;
                    // keep going
                },
                HuffmanNode::Leaf(l) => {
                    // we've reached the bottom
                    return Some(&l.value);
                },
            }
        }
    }

    pub fn decode_one_from_bit_reader<R: Read, const MSB_TO_LSB: bool>(
        &self,
        bit_reader: &mut BitReader<&mut R, MSB_TO_LSB>,
    ) -> Result<Option<&T>, io::Error> {
        let mut current_node = &self.root_node;
        let mut first_iteration = true;
        let mut branching = Vec::new();
        loop {
            let take_true_branch = match bit_reader.read_bit() {
                Ok(Some(b)) => b,
                Ok(None) => return if first_iteration {
                    Ok(None)
                } else {
                    Err(io::ErrorKind::UnexpectedEof.into())
                },
                Err(e) => return Err(e),
            };
            if tracing::enabled!(tracing::Level::DEBUG) {
                if take_true_branch {
                    branching.push(b'1');
                } else {
                    branching.push(b'0');
                }
            }
            first_iteration = false;

            let branch_taken = if take_true_branch {
                &*current_node.true_child
            } else {
                &*current_node.false_child
            };
            match branch_taken {
                HuffmanNode::Branch(b) => {
                    current_node = b;
                },
                HuffmanNode::Leaf(l) => {
                    // we've reached the bottom
                    debug!("Huffman: took branches {}", std::str::from_utf8(&branching).unwrap());
                    branching.reverse();
                    debug!("Huffman: in reverse:   {}", std::str::from_utf8(&branching).unwrap());
                    return Ok(Some(&l.value));
                },
            }
        }
    }
}
impl<T: HuffmanCanonicalizable> HuffmanTree<T> {
    /// Creates a new canonical Huffman tree from the given slice of symbol lengths.
    ///
    /// Symbols are populated according to the implementation of [`HuffmanCanonicalizable`]: the
    /// initial element of `symbol_lengths` is mapped to the value returned by
    /// [`HuffmanCanonicalizable::first_value()`] and the subsequent ones by calling
    /// [`HuffmanCanonicalizable::incremented()`] in turn.
    ///
    /// For built-in primitive integer types, the initial element is mapped to 0, the next to 1,
    /// etc.
    ///
    /// A 0 value in `symbol_lengths` is a "skip", i.e. the corresponding symbol shall not be
    /// encodable through the resulting Huffman tree.
    pub fn new_canonical(symbol_lengths: &[usize]) -> Result<Self, HuffmanConstructionError> {
        // at least one length; all lengths greater than zero
        assert!(symbol_lengths.len() > 0);

        // convert to pairs of symbol and symbol length
        let mut lengths_and_symbols = Vec::with_capacity(symbol_lengths.len());
        let mut current_symbol = T::first_value();
        for &symbol_length in &symbol_lengths[..symbol_lengths.len()-1] {
            let next_symbol = current_symbol.incremented();
            assert!(next_symbol > current_symbol);
            if symbol_length > 0 {
                lengths_and_symbols.push((symbol_length, current_symbol));
            }
            current_symbol = next_symbol;
        }
        // add the final one without incrementing once too many
        let last_symbol_length = *symbol_lengths.last().unwrap();
        if last_symbol_length > 0 {
            lengths_and_symbols.push((last_symbol_length, current_symbol));
        }

        // re-sort by symbol length, then by value
        lengths_and_symbols.sort_unstable();

        // construct the tree
        let mut sequence_to_symbol: BTreeMap<Vec<bool>, T> = BTreeMap::new();

        let max_symbol_length = symbol_lengths.iter().max().copied().unwrap_or(0);
        let mut current_sequence: Vec<bool> = Vec::with_capacity(max_symbol_length);
        let mut iterator = lengths_and_symbols.into_iter();

        // first symbol: given length, all zeroes
        let (first_length, first_symbol) = iterator.next().unwrap();
        while current_sequence.len() < first_length {
            current_sequence.push(false);
        }
        sequence_to_symbol.insert(current_sequence.clone(), first_symbol);

        while let Some((next_length, next_symbol)) = iterator.next() {
            // increment
            increment_bools(&mut current_sequence);

            // extend the sequence if necessary
            while current_sequence.len() < next_length {
                current_sequence.push(false);
            }

            // store
            sequence_to_symbol.insert(current_sequence.clone(), next_symbol);
        }

        // and we're done
        HuffmanTree::new(sequence_to_symbol)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum HuffmanNode<T> {
    Branch(BranchNode<T>),
    Leaf(LeafNode<T>),
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct BranchNode<T> {
    true_child: Box<HuffmanNode<T>>,
    false_child: Box<HuffmanNode<T>>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct LeafNode<T> {
    value: T,
}
impl<T> LeafNode<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum HuffmanNodeUnderConstruction<T> {
    Branch(BranchNodeUnderConstruction<T>),
    Leaf(LeafNode<T>),
}
impl<T> HuffmanNodeUnderConstruction<T> {
    pub const fn as_branch(&self) -> Option<&BranchNodeUnderConstruction<T>> {
        match self {
            Self::Branch(b) => Some(b),
            _ => None,
        }
    }
    pub const fn as_branch_mut(&mut self) -> Option<&mut BranchNodeUnderConstruction<T>> {
        match self {
            Self::Branch(b) => Some(b),
            _ => None,
        }
    }

    pub const fn as_leaf(&self) -> Option<&LeafNode<T>> {
        match self {
            Self::Leaf(l) => Some(l),
            _ => None,
        }
    }

    pub const fn as_leaf_mut(&mut self) -> Option<&mut LeafNode<T>> {
        match self {
            Self::Leaf(l) => Some(l),
            _ => None,
        }
    }

    pub const fn is_finished(&self) -> bool {
        match self {
            Self::Branch(b) => b.is_finished(),
            Self::Leaf(_) => true,
        }
    }
}
impl<T> TryFrom<HuffmanNodeUnderConstruction<T>> for HuffmanNode<T> {
    type Error = HuffmanNodeUnderConstruction<T>;

    fn try_from(value: HuffmanNodeUnderConstruction<T>) -> Result<Self, Self::Error> {
        match value {
            HuffmanNodeUnderConstruction::Leaf(l) => Ok(HuffmanNode::Leaf(l)),
            HuffmanNodeUnderConstruction::Branch(b) => {
                match b.try_into() {
                    Ok(bn) => Ok(HuffmanNode::Branch(bn)),
                    Err(bnuc) => Err(HuffmanNodeUnderConstruction::Branch(bnuc)),
                }
            },
        }
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct BranchNodeUnderConstruction<T> {
    true_child: Option<Box<HuffmanNodeUnderConstruction<T>>>,
    false_child: Option<Box<HuffmanNodeUnderConstruction<T>>>,
}
impl<T> BranchNodeUnderConstruction<T> {
    pub const fn new() -> Self {
        Self {
            true_child: None,
            false_child: None,
        }
    }

    pub const fn is_finished(&self) -> bool {
        let true_child_is_finished = match self.true_child.as_ref() {
            Some(tc) => tc.is_finished(),
            None => false,
        };
        if !true_child_is_finished {
            return false;
        }

        let false_child_is_finished = match self.false_child.as_ref() {
            Some(fc) => fc.is_finished(),
            None => false,
        };
        false_child_is_finished
    }
}
impl<T> TryFrom<BranchNodeUnderConstruction<T>> for BranchNode<T> {
    type Error = BranchNodeUnderConstruction<T>;

    fn try_from(value: BranchNodeUnderConstruction<T>) -> Result<Self, Self::Error> {
        if value.is_finished() {
            let true_child_uc = value.true_child.unwrap();
            let false_child_uc = value.false_child.unwrap();

            let true_child: Box<HuffmanNode<T>> = match (*true_child_uc).try_into() {
                Ok(tc) => Box::new(tc),
                Err(_) => unreachable!(),
            };
            let false_child: Box<HuffmanNode<T>> = match (*false_child_uc).try_into() {
                Ok(fc) => Box::new(fc),
                Err(_) => unreachable!(),
            };

            Ok(Self {
                true_child,
                false_child,
            })
        } else {
            Err(value)
        }
    }
}

/// A value that can be used to generate symbols for a canonical Huffman table.
///
/// [`first_value()`](HuffmanCanonicalizable::first_value) is to return the value for the first
/// symbol in the Huffman table; [`self.incremented()`](HuffmanCanonicalizable::incremented) then
/// derives the next symbol. An invariant is that `self.incremented()` must be strictly greater than
/// `self`; otherwise, [`HuffmanTree::new_canonical()`] panics.
///
/// `HuffmanCanonicalizable` is already implemented for the built-in primitive integer types (`u8`
/// through `u128`, `usize`, `i8` through `i128`, and `isize`), with `first_value()` always
/// returning 0 and `n.incremented()` returning `n + 1`.
pub trait HuffmanCanonicalizable : Ord {
    fn first_value() -> Self;
    fn incremented(&self) -> Self;
}
macro_rules! implement_num_canon {
    ($type:ty) => {
        impl HuffmanCanonicalizable for $type {
            fn first_value() -> Self { 0 }
            fn incremented(&self) -> Self { *self + 1 }
        }
    };
    ($type:ty $(, $additional_type:ty)* $(,)?) => {
        implement_num_canon!($type);
        $(
            implement_num_canon!($additional_type);
        )*
    };
}
implement_num_canon!(u8, u16, u32, u64, u128, usize);
implement_num_canon!(i8, i16, i32, i64, i128, isize);


fn increment_bools(bools: &mut Vec<bool>) {
    for b in bools.iter_mut().rev() {
        if *b {
            *b = false;
            // keep going
        } else {
            *b = true;
            // done
            return;
        }
    }

    // carry fell out; insert true at beginning
    bools.insert(0, true);
}


#[cfg(test)]
mod tests {
    use super::HuffmanTree;
    use std::collections::BTreeMap;

    #[test]
    fn test_tree_construction() {
        let mut mappings = BTreeMap::new();
        mappings.insert(vec![true, true], 'A');
        mappings.insert(vec![false], 'B');
        mappings.insert(vec![true, false, true], 'C');
        mappings.insert(vec![true, false, false], 'D');

        HuffmanTree::new(mappings).unwrap();
    }

    #[test]
    fn test_canonical_tree_construction() {
        let lengths = [
            2, // A/0
            1, // B/1
            3, // C/2
            3, // D/2
        ];
        let canonical_tree: HuffmanTree<u8> = HuffmanTree::new_canonical(&lengths).unwrap();

        let mut manual_mappings = BTreeMap::new();
        manual_mappings.insert(vec![true, false], 0);
        manual_mappings.insert(vec![false], 1);
        manual_mappings.insert(vec![true, true, false], 2);
        manual_mappings.insert(vec![true, true, true], 3);
        let manual_tree = HuffmanTree::new(manual_mappings).unwrap();

        assert_eq!(canonical_tree, manual_tree);
    }
}
