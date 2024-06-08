import subprocess
import time

for n_nodes in [2] + list(range(2, 9)):
    for exp in range(7):
        while True:
            n_tasks = 10 ** exp
            followers = []
            for i in range(1, n_nodes):
                proc = subprocess.Popen(["./target/release/eval", str(n_nodes), str(i), str(n_tasks)])
                followers.append(proc)
            time.sleep(0.5)

            leader = subprocess.run(["./target/release/eval", str(n_nodes), "0", str(n_tasks)], capture_output=True, text=True)
            for proc in followers:
                proc.terminate()
            if leader.returncode == 0:
                print(n_nodes, n_tasks, leader.stdout.strip())
                break
            time.sleep(3.0)
        
