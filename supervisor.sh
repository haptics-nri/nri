#!/usr/bin/env bash

FILE=keepalive

cd `dirname $0`

if [ -e $FILE ]; then
    exit
fi
touch $FILE

while [ -e $FILE ]; do
    screen -Dm -S nri ./run.sh
done

