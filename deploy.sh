#!/bin/env bash

cargo build --release

if [ -z "$DEPLOY_HOST" ]; then
    echo "DEPLOY_HOST is not set"
    exit 1
fi

SSH=root@$DEPLOY_HOST

ssh $SSH systemctl disable testllm
ssh $SSH systemctl stop testllm

scp ./target/release/testllm $SSH:/root/
scp ./testllm.service $SSH:/etc/systemd/system/

ssh $SSH systemctl daemon-reload
ssh $SSH systemctl enable testllm
ssh $SSH systemctl start testllm
