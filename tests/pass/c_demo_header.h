/* Real C Header parsed directly by Axon v7.0 @cImport with zero libclang */
#ifndef C_DEMO_HEADER_H
#define C_DEMO_HEADER_H

#define C_MAGIC_PORT 8443
#define C_FLAG_FAST  (1 << 5)
#define C_HEX_TOKEN  0xCAFE

typedef struct CWindowDesc {
    int width;
    int height;
} CWindowDesc;

int abs(int x);
unsigned long long strlen(const char* s);

#endif
