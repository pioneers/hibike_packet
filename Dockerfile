FROM ubuntu:xenial
LABEL maintainer="brjohnstone@berkeley.edu"

ENV LC_ALL="en_US.UTF-8" LANG="en_US.UTF-8" LANGUAGE="en_US:en"
RUN apt-get update && apt-get install -y locales
RUN locale-gen "en_US.UTF-8"
RUN update-locale 
RUN apt-get update && apt-get install -y build-essential software-properties-common curl

ENV HOME="/home/ubuntu"
WORKDIR /home/ubuntu/
RUN curl https://sh.rustup.rs -sSf > rustup.sh
RUN chmod +x rustup.sh
RUN bash rustup.sh -y
ENV PATH="$PATH:$HOME/.cargo/bin"
# Make sure that we can find cargo
RUN cargo --version

RUN add-apt-repository -y ppa:deadsnakes/ppa
RUN apt-get update -y && apt-get upgrade -y
RUN apt-get install -y python3.7 python3.7-dev
RUN curl https://bootstrap.pypa.io/get-pip.py -sSf > get-pip.py

RUN python3.7 get-pip.py
RUN python3.7 -m pip install pipenv
WORKDIR /home/ubuntu/build-wheel
ADD . /home/ubuntu/build-wheel/
RUN python3.7 -m pipenv install --dev

