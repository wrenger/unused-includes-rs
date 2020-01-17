#include "Classes.hpp"
#include "Functions.hpp"
#include "Ignore_impl.hpp"
#include "InsideMacro.hpp" // keep
#include "Macros.hpp"
#include "TemplClasses.hpp"
#include "TemplFunctions.hpp"
#include "TemplParam.hpp"

template <class T> void templParam() {}

#define INSIDE_MACRO(x)                                                        \
    InsideMacro { x }

int main(int argc, char const *argv[]) {
    functions();
    templFunctions<int>();

    Classes classes;
    TemplClasses<Classes> templClasses;

    templParam<TemplParam>();

    return macros(0);
}
