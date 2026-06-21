import subprocess
import json
import time
import shutil
import os
import sys

def rotate_logs():
    log_dir = ".jules/guardian_logs"
    if not os.path.exists(log_dir):
        os.makedirs(log_dir)

    for log_file in ["guardian_report.json", "guardian_report.md"]:
        if os.path.exists(log_file):
            timestamp = time.strftime("%Y%m%d-%H%M%S")
            archive_name = os.path.join(log_dir, f"{timestamp}_{log_file}")
            shutil.move(log_file, archive_name)

def run_cppcheck():
    # Run cppcheck and look for specific styles or warnings that represent our security rules
    # It catches uninitialized arrays and missing pointers inherently if configured strictly, but our regex was very specific.
    # We will use cppcheck for the AST robustness but it might not perfectly map to our custom framework's 'invoke callback without nullptr check' natively without a custom rule.
    pass

# We will actually keep the regex python script for now as it directly targets the unique callback structure of this framework which cppcheck does not natively flag out-of-the-box without custom rules.
