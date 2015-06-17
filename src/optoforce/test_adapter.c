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
        printf("connected! waiting for data...\n");
        sleep(1);
        lofa_xyz xyz = lofa_sensor_read(sensor);
        printf("x = %g, y = %g, z = %g\n", xyz.x, xyz.y, xyz.y);
        lofa_sensor_disconnect(sensor, true);
    } else {
        printf("failed to connect!\n");
    }
    lofa_free_sensor(sensor);

    return 0;
}

