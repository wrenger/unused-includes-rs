#pragma once

class InsideMacro {
  public:
    int mem;

    explicit InsideMacro(int mem) : mem(mem) {}
    ~InsideMacro() {}
};
