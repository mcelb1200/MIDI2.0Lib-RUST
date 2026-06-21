import os
import re
import sys
import json
import time
import shutil
import subprocess

def get_files_by_ext(exts, directories):
    files = []
    for d in directories:
        if not os.path.exists(d):
            continue
        for root, _, filenames in os.walk(d):
            for filename in filenames:
                if any(filename.endswith(ext) for ext in exts):
                    files.append(os.path.join(root, filename))
    return files

def check_printf(files):
    violations = []
    for f in files:
        with open(f, 'r') as file:
            for i, line in enumerate(file):
                stripped = line.strip()
                if stripped.startswith('//'):
                    continue
                if re.search(r'\bprintf\s*\(', stripped):
                    violations.append({
                        "file": f,
                        "line": i+1,
                        "type": "Information Leakage",
                        "message": "Hardcoded printf found. Use configurable logging instead."
                    })
    return violations

def get_callbacks(headers):
    callbacks = set()
    pattern = re.compile(r'std::function<[^>]+>\s+([a-zA-Z0-9_]+)')
    for f in headers:
        with open(f, 'r') as file:
            content = file.read()
            for match in pattern.finditer(content):
                callbacks.add(match.group(1))
    return callbacks

def check_callbacks(files, callbacks):
    violations = []
    for f in files:
        if not f.endswith('.cpp'):
            continue
        with open(f, 'r') as file:
            content = file.read()
            lines = content.split('\n')

            for i, line in enumerate(lines):
                for cb in callbacks:
                    if re.search(r'\b' + cb + r'\s*\(', line):
                        if "std::function" in line or "inline void" in line:
                            continue

                        stripped = line.strip()
                        if stripped.startswith('//'):
                            continue

                        idx_comment = line.find('//')
                        idx_cb = line.find(cb)
                        if idx_comment != -1 and idx_comment < idx_cb:
                            continue

                        found_check = False

                        for j in range(i, max(-1, i-150), -1):
                            prev_line = lines[j]

                            if re.search(r'\b' + cb + r'\s*!=\s*nullptr|\bif\s*\(\s*' + cb + r'\b|\b' + cb + r'\s*&&', prev_line):
                                found_check = True
                                break
                            if j > 0 and re.search(r'\b' + cb + r'\s*!=\s*', lines[j-1]) and "nullptr" in prev_line:
                                found_check = True
                                break

                            if "{" in prev_line and "switch" not in prev_line and "case" not in prev_line and "if " not in prev_line and "else if" not in prev_line:
                                if "void " in prev_line and "::" in prev_line:
                                    break

                        if not found_check:
                            violations.append({
                                "file": f,
                                "line": i+1,
                                "type": "Denial of Service (DoS)",
                                "message": f"Callback '{cb}' invoked without preceding != nullptr check."
                            })
    return violations

def run_cppcheck():
    violations = []
    try:
        # Run cppcheck for robust uninitialized array and generic C++20 checks
        cmd = [
            "cppcheck",
            "--enable=warning,style,performance,portability",
            "--inconclusive",
            "--std=c++20",
            "-Iinclude",
            "src/midiCIProcessor.cpp",
            "src/umpProcessor.cpp",
            "--suppress=uninitMemberVar",
            "--suppress=unusedFunction",
            "--suppress=unusedPrivateFunction",
            "--suppress=knownConditionTrueFalse",
            "--template={file}:{line}:{severity}:{message}"
        ]

        result = subprocess.run(cmd, capture_output=True, text=True)
        # cppcheck outputs to stderr
        output = result.stderr

        for line in output.split('\n'):
            parts = line.split(':', 3)
            if len(parts) >= 4:
                file_path = parts[0]
                line_num = parts[1]
                severity = parts[2]
                message = parts[3].strip()

                # We specifically look for uninitialized variables or critical style warnings
                # that indicate a security regression.
                if "uninit" in severity.lower() or "uninitialized" in message.lower():
                    violations.append({
                        "file": file_path,
                        "line": int(line_num) if line_num.isdigit() else 0,
                        "type": "Uninitialized Stack Memory (Cppcheck)",
                        "message": message
                    })
    except Exception as e:
        print(f"Error running cppcheck: {e}")

    return violations

def rotate_logs():
    log_dir = ".jules/guardian_logs"
    if not os.path.exists(log_dir):
        os.makedirs(log_dir)

    for log_file in ["guardian_report.json", "guardian_report.md"]:
        if os.path.exists(log_file):
            timestamp = time.strftime("%Y%m%d-%H%M%S")
            archive_name = os.path.join(log_dir, f"{timestamp}_{log_file}")
            shutil.move(log_file, archive_name)

def main():
    directories = ['src', 'include']
    cpp_files = get_files_by_ext(['.cpp', '.h'], directories)
    headers = get_files_by_ext(['.h'], directories)

    callbacks = get_callbacks(headers)

    violations = []
    # Fast regex checks
    violations.extend(check_printf(cpp_files))
    violations.extend(check_callbacks(cpp_files, callbacks))

    # Robust cppcheck integration for C++20 standard
    violations.extend(run_cppcheck())

    # Generate JSON output
    with open("guardian_report.json", "w") as jf:
        json.dump(violations, jf, indent=2)

    # Generate MD output
    with open("guardian_report.md", "w") as mdf:
        mdf.write("# 🛡️ Sentinel Guardian Report\n\n")
        if violations:
            mdf.write("## 🚨 Violations Found\n\n")
            for v in violations:
                mdf.write(f"- **{v['type']}** at `{v['file']}:{v['line']}`\n")
                mdf.write(f"  - {v['message']}\n")
        else:
            mdf.write("## ✅ No Violations Found\n")
            mdf.write("The codebase is secure against tracked patterns.\n")

    if violations:
        print("🛡️ Sentinel Guardian found security violations! Check guardian_report.md")
        sys.exit(1)
    else:
        print("🛡️ Sentinel Guardian: No security violations found.")
        rotate_logs()
        sys.exit(0)

if __name__ == '__main__':
    main()
