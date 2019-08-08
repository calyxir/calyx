#include <vector>

using std::vector;

void print_float_vec(std::vector<float> &input) {
	for (int i = 0; i < input.size(); i++) {
		std::cout << input.at(i) << '\t';
	}
  std::cout << '\n';
}
