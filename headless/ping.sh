#!/usr/bin/env bash

SERVER=https://alexburka.com/ping.php

logger -s "ping \"$1\" \"$2\""

case "$2" in
    dhcp4-change|up)
        while true; do
            logger -s "pinging $SERVER..."
            curl -s --data 'pw=proton' "$SERVER"
            if [ $? -eq 0 ]; then
                logger -s "success!"
                break
            fi
            sleep 1
        done
        ;;
    *)
        ;;
esac

