import matplotlib.pyplot as plt
import numpy as np

nodes = {i: [] for i in range(2, 9)}

for line in open("data2"):
    n, _, t = line.strip().split()
    nodes[int(n)].append(float(t))

x = np.logspace(0, 6, 7, base=10)

for i in range(2, 9):
    print(nodes[i] / x)
    plt.plot(x, nodes[i], label=f"{i} Nodes")

plt.ylabel("Total execution time (s)")
plt.xlabel("Number of tasks")
plt.xscale('log')
plt.yscale('log')
plt.legend()
plt.show()
