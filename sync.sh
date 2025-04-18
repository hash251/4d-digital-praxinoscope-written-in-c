#!/bin/bash
LOCAL_PATH=$(realpath ".")
REMOTE_USER="pi"
REMOTE_IPS=("172.17.21.2" "172.17.21.3" "172.17.21.4" "172.17.21.5")
REMOTE_PATH="/home/pi/programming/project"
LOCAL_IP=$(hostname -I | awk '{print $1}')

for REMOTE_IP in "${REMOTE_IPS[@]}"; do
  echo "=========================================="
  echo "[+] Starting sync process for $REMOTE_IP"
  echo "=========================================="
  
  if ! ping -c 1 -W 2 $REMOTE_IP >/dev/null 2>&1; then
    echo "[!] ERROR: Cannot reach $REMOTE_IP - skipping this host"
    echo "=========================================="
    continue
  fi
  
  if ! ssh -o ConnectTimeout=10 -o BatchMode=yes "$REMOTE_USER@$REMOTE_IP" "echo SSH connection successful" >/dev/null 2>&1; then
    echo "[!] ERROR: Cannot establish SSH connection to $REMOTE_IP - skipping this host"
    echo "=========================================="
    continue
  fi
  
  echo "[+] Stopping any running Electron processes on remote machine"
  ssh -o ConnectTimeout=10 "$REMOTE_USER@$REMOTE_IP" "pkill -f electron || true"
  echo "[+] All Electron processes stopped"
  
  echo "[+] Creating directory structure on remote machine"
  ssh -o ConnectTimeout=10 "$REMOTE_USER@$REMOTE_IP" "mkdir -p $REMOTE_PATH"
  
  ssh -o ConnectTimeout=10 "$REMOTE_USER@$REMOTE_IP" "if [ -d \"$REMOTE_PATH\" ] && [ \"\$(ls -A $REMOTE_PATH 2>/dev/null)\" ]; then find $REMOTE_PATH -mindepth 1 -maxdepth 1 ! -name 'node_modules' -exec rm -rf {} \; ; fi"
  echo "[+] Remote directory cleaned (except for node_modules if it exists)"
  
  rsync -av --timeout=10 "$LOCAL_PATH/" "$REMOTE_USER@$REMOTE_IP:$REMOTE_PATH" --exclude 'node_modules' --exclude 'target' --exclude '.git'
  echo "Sync completed: Local → Remote ($REMOTE_USER@$REMOTE_IP:$REMOTE_PATH)"
  
  ssh -o ConnectTimeout=10 "$REMOTE_USER@$REMOTE_IP" "echo 'IP=$LOCAL_IP' > $REMOTE_PATH/.env &&
   echo 'PORT=1337' >> $REMOTE_PATH/.env &&
   echo 'PROTOCOL=ws' >> $REMOTE_PATH/.env"
  echo "[+] .env file updated on remote with the local ip: $LOCAL_IP"
  
  # Fix NVM setup and Node version check
  ssh -o ConnectTimeout=10 "$REMOTE_USER@$REMOTE_IP" "bash -c 'export NVM_DIR=\$HOME/.nvm;
   if [ ! -s \"\$NVM_DIR/nvm.sh\" ]; then
     echo \"nvm is not installed. Installing nvm...\";
     curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.2/install.sh | bash;
     export NVM_DIR=\$HOME/.nvm;
     [ -s \"\$NVM_DIR/nvm.sh\" ] && . \"\$NVM_DIR/nvm.sh\";
   else
     . \"\$NVM_DIR/nvm.sh\";
   fi;
   
   NODE_VERSION=\$(node --version 2>/dev/null | cut -d \"v\" -f 2);
   if [ -z \"\$NODE_VERSION\" ]; then
     echo \"[+] Node.js not found. Installing version 22...\";
     nvm install 22;
     nvm use 22;
     echo \"[+] Node.js installed: \$(node --version)\";
   else
     NODE_MAJOR=\${NODE_VERSION%%.*};
     if [ \"\$NODE_MAJOR\" -lt 20 ]; then
       echo \"[+] Node.js version \$(node --version) is less than 20. Upgrading...\";
       nvm install 22;
       nvm use 22;
       echo \"[+] Node.js upgraded to: \$(node --version)\";
     else
       echo \"[+] Node.js version \$(node --version) is compatible.\";
     fi;
   fi'"
  
  # Fix the npm install and dependency setup
  ssh -o ConnectTimeout=10 "$REMOTE_USER@$REMOTE_IP" "bash -c 'export NVM_DIR=\$HOME/.nvm;
   [ -s \"\$NVM_DIR/nvm.sh\" ] && . \"\$NVM_DIR/nvm.sh\";
   cd $REMOTE_PATH;
   echo \"Using Node: \$(node --version)\";
   if [ ! -d \"node_modules\" ]; then
     echo \"[+] node_modules not found, running npm install...\";
     if [ -f package.json ]; then
       if [ -f package-lock.json ]; then
         npm ci || npm install;
       else
         npm install;
       fi;
       echo \"[+] Dependencies installed on remote server\";
     else
       echo \"[!] WARNING: No package.json found in $REMOTE_PATH, skipping npm install\";
     fi;
   else
     echo \"[+] node_modules directory exists, skipping npm install\";
   fi'"
  
  # Launch the electron application
  ssh -X -o ConnectTimeout=10 "$REMOTE_USER@$REMOTE_IP" "bash -c 'export NVM_DIR=\$HOME/.nvm;
   [ -s \"\$NVM_DIR/nvm.sh\" ] && . \"\$NVM_DIR/nvm.sh\";
   cd $REMOTE_PATH;
   if [ -f \"./node_modules/.bin/electron\" ]; then
     DISPLAY=:0 ./node_modules/.bin/electron ./display_client;
     echo \"[+] Started electron application on remote server\";
   else
     echo \"[!] WARNING: Electron not found in node_modules, skipping application start\";
   fi'"
  
  echo "=========================================="
  echo "[+] Completed sync process for $REMOTE_IP"
  echo "=========================================="
done