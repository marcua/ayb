#!/bin/bash

mkdir tests/smtp_data_$1
source tests/smtp_data/test-env/bin/activate || python3 -m venv tests/smtp_data_$1/test-env && source tests/smtp_data_$1/test-env/bin/activate
pip install aiosmtpd
python tests/smtp_server.py tests/smtp_data_$1 $1

