#!/bin/bash

source tests/test-env/bin/activate
mkdir tests/smtp_data_$1
python tests/smtp_server.py tests/smtp_data_$1 $1

