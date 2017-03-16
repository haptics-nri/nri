#!/bin/bash

arg="$1"

case "$arg" in
    "hotspot" )
        sudo /bin/systemctl disable network-manager
        sudo /bin/systemctl enable create_ap
        ;;
    "wifi" )
        sudo /bin/systemctl disable create_ap
        sudo /bin/systemctl enable network-manager
        ;;
    * )
        echo "ERROR: unrecognized network type"
        exit 1
        ;;
esac

sudo /sbin/reboot

