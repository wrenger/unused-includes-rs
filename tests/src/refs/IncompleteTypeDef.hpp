#pragma once

class IncompleteType {
    int i;

  public:
    IncompleteType(int i) : i(i) {}
    IncompleteType() : i(0) {}
    ~IncompleteType() {}
};
