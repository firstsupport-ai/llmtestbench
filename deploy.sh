#!/bin/env bash

cargo build --release

if [ -z "$DEPLOY_HOST" ]; then
    echo "DEPLOY_HOST is not set"
    exit 1
fi

SSH=root@$DEPLOY_HOST

rsync -avz --progress ./target/release/testllm $SSH:/root/
rsync -avz --progress ./testllm.service $SSH:/etc/systemd/system/

ssh $SSH systemctl disable testllm
ssh $SSH systemctl stop testllm

ssh $SSH systemctl daemon-reload
ssh $SSH systemctl enable testllm
ssh $SSH systemctl start testllm
