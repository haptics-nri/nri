#!/usr/bin/env bash

cd `dirname $0`
rm keepalive

screen -S nri -p 0 -X stuff "quit"
sleep 1

