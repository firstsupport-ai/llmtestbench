#!/bin/env bash
cd entity/src
sea generate entity -l --with-serde serialize
cd ../..
