#include "Classes.hpp"
#include "Functions.hpp"
#include "TemplClasses.hpp"
#include "TemplFunctions.hpp"
#include "TemplParam.hpp"
#include "Macros.hpp"

template<class T> void templParam() {}

int main(int argc, char const *argv[]) {
    functions();
    templFunctions<int>();

    Classes classes;
    TemplClasses<Classes> templClasses;

    templParam<TemplParam>();

    return macros(0);
}
