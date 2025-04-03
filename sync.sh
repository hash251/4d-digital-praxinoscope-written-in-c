#!/bin/bash
LOCAL_PATH=$(realpath ".")
REMOTE_USER="pi"
REMOTE_IP="10.107.200.150"
REMOTE_PATH="/home/pi/programming/Test"
LOCAL_IP=$(hostname -I | awk '{print $1}')

echo "[+] Stopping any running Electron processes on remote machine"
ssh "$REMOTE_USER@$REMOTE_IP" "pkill -f electron || true"
echo "[+] All Electron processes stopped"

ssh "$REMOTE_USER@$REMOTE_IP" "find $REMOTE_PATH -mindepth 1 -maxdepth 1 ! -name 'node_modules' -exec rm -rf {} \;"
echo "[+] Remote directory cleaned (except for node_modules)"

rsync -av "$LOCAL_PATH/" "$REMOTE_USER@$REMOTE_IP:$REMOTE_PATH" --exclude 'node_modules' --exclude 'target' --exclude '.git'
echo "Sync completed: Local → Remote ($REMOTE_USER@$REMOTE_IP:$REMOTE_PATH)"
ssh "$REMOTE_USER@$REMOTE_IP" "echo 'IP=$LOCAL_IP' > $REMOTE_PATH/.env && 
                                 echo 'PORT=1337' >> $REMOTE_PATH/.env && 
                                 echo 'PROTOCOL=ws' >> $REMOTE_PATH/.env"
echo "[+] .env file updated on remote with the local ip: $LOCAL_IP"

ssh "$REMOTE_USER@$REMOTE_IP" "bash -c 'export NVM_DIR=\$HOME/.nvm;
  if [ ! -s \"\$NVM_DIR/nvm.sh\" ]; then
    echo \"nvm is not installed. Installing nvm...\"
    curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.2/install.sh | bash
    . \$NVM_DIR/nvm.sh
  else
    . \$NVM_DIR/nvm.sh
  fi
  NODE_VERSION=\$(node --version | cut -d \"v\" -f 2)
  NODE_MAJOR=\${NODE_VERSION%%.*}
  if [ \"\$NODE_MAJOR\" -lt 20 ]; then
    echo \"[+] Node.js version \$(node --version) is less than 20. Upgrading...\"
    nvm install 22
    nvm use 22
    echo \"[+] Node.js upgraded to: \$(node --version)\"
  else
    echo \"[+] Node.js version \$(node --version) is compatible.\"
  fi'"

# npm ci will quickly install exactly what is in package-lock.json if it exists
# ssh "$REMOTE_USER@$REMOTE_IP" "bash -c 'export NVM_DIR=\$HOME/.nvm;
#  . \$NVM_DIR/nvm.sh;
#  cd $REMOTE_PATH;
#  echo \"Using Node: \$(node --version)\";
#  if [ -f package-lock.json ]; then
#    npm ci || npm install;
#  else
#    npm install;
#  fi'"
# echo "Dependencies installed on remote server"

ssh -X "$REMOTE_USER@$REMOTE_IP" "bash -c 'export NVM_DIR=\$HOME/.nvm;
  . \$NVM_DIR/nvm.sh;
  cd $REMOTE_PATH;
  DISPLAY=:0 ./node_modules/.bin/electron ./display_client'"
echo "[+] Started electron application on remote server"
