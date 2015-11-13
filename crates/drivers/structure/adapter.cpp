#include <OpenNI.h>

extern "C"
{
    openni::Device* Device_new();
    void Device_delete(openni::Device *that);
    openni::VideoStream* VideoStream_new();
    void VideoStream_delete(openni::VideoStream *that);

    openni::Status initialize();
    void shutdown();
}

openni::Device* Device_new()
{
    return new openni::Device();
}

void Device_delete(openni::Device *that)
{
    delete that;
}

openni::VideoStream* VideoStream_new()
{
    return new openni::VideoStream();
}

void VideoStream_delete(openni::VideoStream *that)
{
    delete that;
}

openni::Status initialize()
{
    return openni::OpenNI::initialize();
}

void shutdown()
{
    openni::OpenNI::shutdown();
}

