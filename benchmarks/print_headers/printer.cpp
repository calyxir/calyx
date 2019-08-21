#include <vector>
#include <cstdio>

using std::vector;

template <typename T>
void print_vec(std::vector<T> &input) {
	for (uint i = 0; i < input.size(); i++) {
    printf("%.4f\n", input.at(i));
	}
}

template<typename T>
void print_vec_2d(std::vector<std::vector<T>> &input) {
  for (uint i = 0; i < input.size(); i++) {
    print_vec(input.at(i));
  }
}

template<typename T>
void print_vec_3d(std::vector<std::vector<std::vector<T>>> &input) {
  for (uint i = 0; i < input.size(); i++) {
    print_vec_2d(input.at(i));
  }
}
