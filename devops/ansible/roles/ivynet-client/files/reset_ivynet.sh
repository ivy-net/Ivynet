#!/bin/bash
#
IVYHOME=/home/ivynet
IVYPATH=${IVYHOME}/.ivynet
IVYCONF=monitor-config.toml

sudo systemctl stop ivynet-client

sudo -u ivynet  bash -c "[[ -f ${IVYPATH}/${IVYCONF} ]] && cp ${IVYPATH}/${IVYCONF} ${IVYHOME}"
sudo -u ivynet rm -r ${IVYPATH}
sudo -i -u ivynet SERVER_URL=https://api3.test.ivynet.dev:50050 ivynet register-node
sudo -u ivynet cp ${IVYHOME}/${IVYCONF} ${IVYPATH}

sudo systemctl start ivynet-client
