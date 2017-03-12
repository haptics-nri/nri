#!/bin/bash

DATADIR=/mnt/usbstick/proton_data
DATE=20170301
FLOW=optocam

SMOOTH="smooth/rough (1..5)"
SOFT="soft/hard (1..5)"
SLIPPERY="slippery/sticky (1..5)"
COLD="cool/warm (1..5)"

function get_surface {
    rg surface $1/$2.flow | cut -d\" -f4;
}

function output {
    echo "\ \ \ \ > \"$1\" [$2] [$(date +%s.%N)]";
}

PROPS="SLIPPERY"
for i in $DATADIR/$DATE/$FLOW/{?,??}
do
    echo $(basename $i) $(get_surface $i $FLOW)

    for p in $PROPS
    do
        PROP=${!p}

        echo -ne "\t$PROP: "
        read v

        sed -i -e "\$i$(output "$PROP" $v)" $i/$FLOW.flow
    done
done

# how to fix improper quoting
# sed -e 's/> \(.*\) (1..5)/> "\1" (1..5)/' /mnt/usbstick/proton_data/20170222/biocam/1/*.flow

