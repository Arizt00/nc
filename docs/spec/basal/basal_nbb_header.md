# Basal .nbb Header

The `NBB0` header is 64 bytes:

- magic: `NBB0`
- version: `5`
- endian: little
- pointer width: `64`
- section count
- header size
- section table offset
- section table size
- total file size
- flags
- structural checksum

The structural checksum is SHA-256 over the complete file with the checksum field zeroed. This makes the header self-verifiable without depending on external tools.
