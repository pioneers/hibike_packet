#!/bin/bash
# Compile the wheel. Run this on the host machine.
rm -rf build
sudo docker build . -t hibike_packet
sudo docker run --name hibike_packet\
    -v "$(pwd)/build":"/home/ubuntu/build-wheel/artefacts" hibike_packet /bin/bash container_script.sh 


