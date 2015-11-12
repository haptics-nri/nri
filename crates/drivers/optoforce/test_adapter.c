#include <stdio.h>
#include <math.h>
#include "adapter.h"

int main()
{
    lofa_version version = lofa_get_version();
    printf("version: %d.%d.%d\n", version.major, version.minor, version.revision);

    LOFA_SENSOR_HANDLE sensor = lofa_create_sensor(-1, NAN);
    printf("created sensor with handle %p\n", sensor);
    if (lofa_sensor_connect(sensor, "/dev/ttyACM0", -1)) {
        long i;
        printf("connected! waiting for data...\n");
        while (true) {
            lofa_xyz xyz = lofa_sensor_read(sensor);
            printf("x = %.3f, y = %.3f, z = %.3f\n", xyz.x, xyz.y, xyz.z);
            usleep(50000);
        }
        printf("disconnecting\n");
        lofa_sensor_disconnect(sensor, true);
    } else {
        printf("failed to connect!\n");
    }
    lofa_free_sensor(sensor);

    return 0;
}

