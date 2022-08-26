import numpy as np
import sys

mat1 = np.random.rand(1, 64, 55, 55)
mat2 = np.random.rand(1, 64, 55, 55)
mat3 = np.random.rand(1, 128, 55, 55)

np.set_printoptions(threshold=sys.maxsize)
with open("dat1.txt", "w") as file:
    file.writelines(repr(mat1))
with open("dat2.txt", "w") as file:
    file.writelines(repr(mat2))
with open("dat3.txt", "w") as file:
    file.writelines(repr(mat3))

mat4 = np.concatenate((mat1, mat2), axis=1)
with open("dat4.txt", "w") as file:
    file.writelines(repr(mat4))
