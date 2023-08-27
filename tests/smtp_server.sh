#!/bin/bash

mkdir smtp_data
source smtp_data/test-env/bin/activate || python3 -m venv smtp_data/test-env && source smtp_data/test-env/bin/activate
pip install aiosmtpd
python smtp_server.py smtp_data

