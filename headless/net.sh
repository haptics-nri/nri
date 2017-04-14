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
    "" )
        if sudo /bin/systemctl is-enabled -q create_ap; then
            "$0" wifi
        else
            "$0" hotspot
        fi
        ;;
    * )
        echo "ERROR: unrecognized network type"
        exit 1
        ;;
esac

sudo /sbin/reboot

