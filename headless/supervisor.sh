#!/usr/bin/env bash

FILE=keepalive

echo "NRI SUPERVISOR"
echo ""

sudo rfkill unblock 0

if systemctl is-enabled -q network-manager; then
    sudo create_ap --fix-unmanaged
    sudo nmcli c u id AirPennNet
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

