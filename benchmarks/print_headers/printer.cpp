#include <vector>
#include <cstdio>

using std::vector;

void print_float_vec(std::vector<float> &input) {
	for (uint i = 0; i < input.size(); i++) {
    printf("%.4f\n", input.at(i));
	}
}
