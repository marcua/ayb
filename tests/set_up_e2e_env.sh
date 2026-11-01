#!/bin/bash

# Initialize and activate virtual environment
if [ ! -d "tests/test-env" ]; then
    python3 -m venv tests/test-env
fi
source tests/test-env/bin/activate

# Install requirements
# Pin playwright to the version bundled by the playwright-rs crate's driver
# (see PLAYWRIGHT_VERSION in the crate's build.rs) so the Chromium revision
# installed here matches the driver the tests drive it with.
pip install awscli "playwright==1.64.0" "moto[server]==5.2.4"

# Try to install Playwright browsers, but don't fail if unsupported
echo "Installing Playwright browsers..."
if playwright install chromium >/dev/null 2>&1; then
    echo "Playwright browsers installed successfully"
else
    echo "Using system browsers (Playwright browsers not available on this platform)"
fi

# Start the S3 mock
tests/run_s3_mock.sh
