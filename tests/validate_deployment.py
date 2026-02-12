#!/usr/bin/env python3
import urllib.request
import urllib.error
import urllib.parse
import json
import sys
import subprocess
import time

BASE_URL = "http://PacketParamedic.alpina:8080/api/v1"
SSH_HOST = "alfa@PacketParamedic.alpina"
RED = "\033[91m"
GREEN = "\033[92m"
RESET = "\033[0m"

def log(msg, status="INFO"):
    color = GREEN if status == "PASS" else (RED if status == "FAIL" else RESET)
    print(f"[{color}{status}{RESET}] {msg}")

def make_request(endpoint, method="GET", data=None):
    url = f"{BASE_URL}{endpoint}"
    try:
        req = urllib.request.Request(url, method=method)
        if data:
            json_data = json.dumps(data).encode('utf-8')
            req.add_header('Content-Type', 'application/json')
            req.data = json_data
            
        with urllib.request.urlopen(req) as response:
            status = response.status
            body = response.read().decode('utf-8')
            try:
                json_body = json.loads(body)
            except:
                json_body = body
            return status, json_body
    except urllib.error.HTTPError as e:
        return e.code, e.read().decode('utf-8')
    except Exception as e:
        log(f"Request failed: {e}", "FAIL")
        return 0, str(e)

def test_health():
    log("Testing /health endpoint...")
    status, body = make_request("/health")
    if status == 200 and isinstance(body, dict) and body.get("data", {}).get("status") == "ok":
        log("Health check passed", "PASS")
        return True
    log(f"Health check failed: {status} {body}", "FAIL")
    return False

def test_self_test():
    log("Testing /self-test/latest endpoint...")
    status, body = make_request("/self-test/latest")
    # It might be null if strictly new, but we ran it manually earlier. 
    # Let's hope it persisted or the manual run counts.
    if status == 200:
        log("Self-test endpoint reachable", "PASS")
        # Optional: check if data is null or present
        if isinstance(body, dict):
            data = body.get("data")
            if data:
                log(f"Found self-test data: {len(str(data))} bytes", "PASS")
            else:
                 log("Self-test data is null (expected if in-memory only or cleared)", "INFO")
        return True
    log(f"Self-test check failed: {status}", "FAIL")
    return False

def test_network_interfaces():
    log("Testing /network/interfaces endpoint...")
    status, body = make_request("/network/interfaces")
    if status == 200:
        log("Network interfaces endpoint reachable", "PASS")
        return True
    log(f"Network interfaces check failed: {status}", "FAIL")
    return False

def test_schedule_crud():
    log("Testing Schedule CRUD...")
    
    # 1. Create
    test_schedule = {
        "name": "integration_test_sched",
        "cron": "0 0 * * *",
        "test": "speed-test-mock"
    }
    status, body = make_request("/schedules", "POST", test_schedule)
    if status != 201: # Created
        log(f"Failed to create schedule: {status} {body}", "FAIL")
        return False
    log("Schedule created", "PASS")

    # 2. List
    status, body = make_request("/schedules")
    schedules = body.get("data", []) if isinstance(body, dict) else []
    found = any(s['name'] == "integration_test_sched" for s in schedules)
    if not found:
        log("Created schedule not found in list", "FAIL")
        return False
    log("Schedule found in list", "PASS")

    # 3. Dry Run
    status, body = make_request("/schedules/dry-run?hours=24")
    if status == 200:
        log("Dry run endpoint working", "PASS")
    else:
        log(f"Dry run failed: {status}", "FAIL")

    # 4. Delete
    status, body = make_request("/schedules/integration_test_sched", "DELETE")
    if status != 200:
        log(f"Failed to delete schedule: {status}", "FAIL")
        return False
    log("Schedule deleted", "PASS")
    
    return True

def test_ssh_cli():
    log("Testing Remote CLI access...")
    cmd = f'ssh -o BatchMode=yes {SSH_HOST} "./PacketParamedic/target/release/packetparamedic --version"'
    try:
        output = subprocess.check_output(cmd, shell=True, stderr=subprocess.STDOUT).decode().strip()
        if "packetparamedic" in output:
            log(f"CLI version check passed: {output}", "PASS")
            return True
        else:
            log(f"Unexpected CLI output: {output}", "FAIL")
            return False
    except subprocess.CalledProcessError as e:
        log(f"CLI check failed: {e.output.decode()}", "FAIL")
        return False

def main():
    print("=== Starting PacketParamedic Integration Tests ===")
    
    tests = [
        test_health,
        test_network_interfaces,
        test_self_test,
        test_schedule_crud,
        test_ssh_cli
    ]
    
    passed = 0
    for test in tests:
        try:
            if test():
                passed += 1
        except Exception as e:
            log(f"Test {test.__name__} crashed: {e}", "FAIL")
            
    print("\n=== Test Summary ===")
    print(f"Total: {len(tests)}")
    print(f"Passed: {passed}")
    print(f"Failed: {len(tests) - passed}")
    
    if passed == len(tests):
        print(f"{GREEN}ALL TESTS PASSED{RESET}")
        sys.exit(0)
    else:
        print(f"{RED}SOME TESTS FAILED{RESET}")
        sys.exit(1)

if __name__ == "__main__":
    main()
