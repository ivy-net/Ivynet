#!/bin/bash

counter=1

while [ $counter -le 5 ]; do
    echo "Counter: $counter"
    counter=$((counter + 1))
    sleep 5
done
