#!/usr/bin/env bash

FILE=keepalive

cd `dirname $0`

touch $FILE

while [ -e $FILE ]; do
    screen -Dm -S nri ./run.sh
done

