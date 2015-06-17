#ifndef _NRI_LIBOPTOFORCE_ADAPTER_HPP_
#define _NRI_LIBOPTOFORCE_ADAPTER_HPP_

typedef struct
{
    unsigned major;
    unsigned minor;
    unsigned revision;
} lofa_version;

typedef struct
{
    double x;
    double y;
    double z;
} lofa_xyz;

#ifdef __cplusplus

    #include <Sensor.hpp>

    // stolen from dump_readings.cpp
    // TODO experiment to see if buffer size affects data rate
    #define DEFAULT_BUFFER      32
    #define DEFAULT_FACTOR      0.001
    #define DEFAULT_BAUDRATE    115200

    typedef optoforce::Sensor* LOFA_SENSOR_HANDLE;

    extern "C"
    {
#else
    typedef unsigned char bool;
    #define true 1
    #define false 0
    typedef void* LOFA_SENSOR_HANDLE;
#endif

    lofa_version lofa_get_version(void);
    LOFA_SENSOR_HANDLE lofa_create_sensor(int buffer, float factor);
    bool lofa_sensor_connect(LOFA_SENSOR_HANDLE that, const char *device, int baudrate);
    void lofa_sensor_disconnect(LOFA_SENSOR_HANDLE that, bool should_block);
    void lofa_free_sensor(LOFA_SENSOR_HANDLE that);
    lofa_xyz lofa_sensor_read(const LOFA_SENSOR_HANDLE that);

#ifdef __cplusplus
    }
#endif

#endif // _NRI_LIBOPTOFORCE_ADAPTER_HPP_

