#!/usr/bin/env bash

FILE=keepalive
SERVER=https://alexburka.com/ping.php

echo "NRI SUPERVISOR"
echo ""

if systemctl is-enabled -q network-manager; then
    sudo create_ap --fix-unmanaged
    sudo nmcli c u id AirPennNet
    while true; do
        echo "pinging $SERVER..."
        curl -s --data 'pw=proton' "$SERVER"
        if [ $? -eq 0 ]; then
            echo "success!"
            break
        fi
        sleep 1
    done
fi

echo "checking $FILE..."

cd `dirname $0`

if [ -e $FILE ]; then
    echo "keepalive already present. goodbye!"
    exit
fi

echo "creating keepalive"
touch $FILE

while [ -e $FILE ]; do
    echo "starting in screen..."
    screen -L -Dm -S nri ./run.sh
done

echo "keepalive deleted. goodbye!"

