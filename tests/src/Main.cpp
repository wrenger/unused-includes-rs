#include "Base.hpp"
#include "Sub.hpp"
#include "Unused.hpp"
#include "External.hpp"

int main(int argc, char const *argv[])
{
    Sub sub = { 5, Base() };
    return sub.val;
}
