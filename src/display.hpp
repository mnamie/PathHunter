#pragma once
#include <ostream>
#include <vector>
#include "audit.hpp"
#include "args.hpp"

void render(std::ostream& out, const std::vector<PathEntry>& entries, const Config& cfg);
