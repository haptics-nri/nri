// g++ test.cpp -I../../../bluefox/driver -L. -lmvDeviceManager && ./a.out

#include <mvDeviceManager/Include/mvDeviceManager.h>
#include <cstdio>

using namespace std;

#define print_struct(s) printf(#s ": %lu bytes (aligned at %lu)\n", sizeof(s), __alignof__(s))
#define print_member(s,m) printf("\t" #m ": %lu\n", __builtin_offsetof(s, m))

int main()
{
    print_struct(ImageBuffer);
    print_member(ImageBuffer, iBytesPerPixel);
    print_member(ImageBuffer, iHeight);
    print_member(ImageBuffer, iWidth);
    print_member(ImageBuffer, pixelFormat);
    print_member(ImageBuffer, iSize);
    print_member(ImageBuffer, vpData);
    print_member(ImageBuffer, iChannelCount);
    print_member(ImageBuffer, pChannels);
    print_struct(ChannelData);
    print_member(ChannelData, iChannelOffset);
    print_member(ChannelData, iLinePitch);
    print_member(ChannelData, iPixelPitch);
    print_member(ChannelData, szChannelDesc);
    return 0;
}

