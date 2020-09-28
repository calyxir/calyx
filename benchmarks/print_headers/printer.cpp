#include <vector>
#include <cstdio>

template <typename T>
void print_vec(const std::vector<T> &input) {
  for (const auto i : input) {
    printf("%.4f\n", i);
  }
}

template<typename T>
void print_vec_2d(const std::vector<std::vector<T>> &input) {
  for (const auto &1d_vector : input) {
    print_vec(1d_vector);
  }
}

template<typename T>
void print_vec_3d(const std::vector<std::vector<std::vector<T>>> &input) {
  for (const auto &2d_vector : input) {
    print_vec_2d(2d_vector);
  }
}
