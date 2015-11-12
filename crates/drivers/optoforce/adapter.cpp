#include "adapter.h"

using namespace optoforce;

lofa_version lofa_get_version(void)
{
    return {.major = 0, .minor = 0, .revision = 1};
}

LOFA_SENSOR_HANDLE lofa_create_sensor(int buffer, float factor)
{
    if (buffer == -1) buffer = DEFAULT_BUFFER;
    if (isnan(factor)) factor = DEFAULT_FACTOR;

    return new Sensor(buffer, factor);
}

bool lofa_sensor_connect(LOFA_SENSOR_HANDLE that, const char *device, int baudrate)
{
    if (baudrate == -1) baudrate = DEFAULT_BAUDRATE;

    that->connect(device, baudrate);
    return that->isConnected();
}

void lofa_sensor_disconnect(LOFA_SENSOR_HANDLE that, bool should_block)
{
    that->disconnect(should_block);
}

void lofa_free_sensor(LOFA_SENSOR_HANDLE that)
{
    // TODO thread-locally store a list of valid pointers to avoid crashing?
    delete that;
}

lofa_xyz lofa_sensor_read(const LOFA_SENSOR_HANDLE that)
{
    while (!that->hasPackages());
    SensorPackage package = that->getPackage();
    SensorReading reading = that->getReading(Sensor::buffer_position_newest, false);
    return {.x = reading.getForceX(), .y = reading.getForceY(), .z = reading.getForceZ()};
}

unsigned char lofa_sensor_get(LOFA_SENSOR_HANDLE that)
{
    while (!that->hasPackages());
    SensorPackage package = that->getPackage();
    SensorConfig config = package.getConfig();
    return config.toByte();
}

void lofa_sensor_set(LOFA_SENSOR_HANDLE that, unsigned char byte)
{
    that->configure(*((SensorConfig*)&byte));
}

