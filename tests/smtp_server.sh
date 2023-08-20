#!/bin/bash

source test-env/bin/activate || python3 -m venv test-env && source test-env/bin/activate
pip install aiosmtpd
python smtp_server.py $1

