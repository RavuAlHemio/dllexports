# Reformatted "E8 Call Translation" pseudocode

Taken from [[MS-PATCH] § 2.2.2](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-patch/e6c8366d-a8c3-4f34-aa5d-56f44f938a09).

First block:

```
if (( chunk_offset < 0x40000000 ) && ( chunk_size > 10 ))
    for ( i = 0; i < (chunk_size – 10); i++ )
        if ( chunk_byte[ i ] == 0xE8 )
            long current_pointer = chunk_offset + i;
            long displacement =
                chunk_byte[ i+1 ]       |
                chunk_byte[ i+2 ] << 8  |
                chunk_byte[ i+3 ] << 16 |
                chunk_byte[ i+4 ] << 24;
            long target = current_pointer + displacement;
            if (( target >= 0 ) && ( target < E8_file_size+current_pointer))
                if ( target >= E8_file_size )
                    target = displacement – E8_file_size;
                endif
                chunk_byte[ i+1 ] = (byte)( target );
                chunk_byte[ i+2 ] = (byte)( target >> 8 );
                chunk_byte[ i+3 ] = (byte)( target >> 16 );
                chunk_byte[ i+4 ] = (byte)( target >> 24 );
            endif
            i += 4;
        endif
    endfor
endif
```

Second block:

```
long value =
    chunk_byte[ i+1 ]       |
    chunk_byte[ i+2 ] << 8  |
    chunk_byte[ i+3 ] << 16 |
    chunk_byte[ i+4 ] << 24;

if (( value >= -current_pointer ) && ( value < E8_file_size ))
    if ( value >= 0 )
        displacement = value – current_pointer;
    else
        displacement = value + E8_file_size;
    endif
    chunk_byte[ i+1 ] = (byte)( displacement );
    chunk_byte[ i+2 ] = (byte)( displacement >> 8 );
    chunk_byte[ i+3 ] = (byte)( displacement >> 16 );
    chunk_byte[ i+4 ] = (byte)( displacement >> 24 );
endif
```
