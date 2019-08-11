#include <vector>
#include <cstdio>

using std::vector;

void print_float_vec(std::vector<float> &input) {
	for (uint i = 0; i < input.size(); i++) {
    printf("%.4f\t", input.at(i));
	}
  printf("\n");
}
