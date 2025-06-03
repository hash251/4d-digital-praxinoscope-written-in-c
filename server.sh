#!/bin/bash

# used when starting the node app from systemctl

export NVM_DIR="/home/softdev/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"  # Load nvm
cd /home/softdev/programming/project
npm start
