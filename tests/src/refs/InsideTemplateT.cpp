#include "InsideTemplate.hpp"

template <typename T> class Template {
    InsideTemplate i;

  public:
    Template() {}
    ~Template() {}
};

int main(int argc, char const *argv[])
{
    Template<int> temp;
    return 0;
}
