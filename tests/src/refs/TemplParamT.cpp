#include "TemplParam.hpp"

template <class T> void templParam() {}

int main(int argc, char const *argv[]) {
    templParam<TemplParam>();
    return 0;
}
