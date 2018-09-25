#!/bin/bash
# Compile the wheel. Run this on the host machine.
sudo rm -rf build
sudo docker rm hibike_packet
sudo docker build . -t hibike_packet
sudo docker run --name hibike_packet\
    -v "$(pwd)/build":"/home/ubuntu/build-wheel/artefacts" hibike_packet /bin/bash container_script.sh 


