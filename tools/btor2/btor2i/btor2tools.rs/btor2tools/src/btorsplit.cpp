/**
 *  Btor2Tools: A tool package for the BTOR format.
 *
 *  Copyright (c) 2019 Mathias Preiner.
 *
 *  All rights reserved.
 *
 *  This file is part of the Btor2Tools package.
 *  See LICENSE.txt for more information on using this software.
 */

#include <sys/stat.h>
#include <algorithm>
#include <cstdarg>
#include <cstring>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <unordered_set>
#include <vector>

/*--------------------------------------------------------------------------*/

static uint32_t s_verbosity = 0;

#define Log ((s_verbosity) ? std::cout : {})

static void
print_usage()
{
  std::cout << "Usage:" << std::endl;
  std::cout << "  btorsplit [options] BTOR2_FILE...\n" << std::endl;
  std::cout << "Options:" << std::endl;
  std::cout << "  -h,--help   Print this help and exit." << std::endl;
  std::cout << "  -v          Increase verbosity." << std::endl;
  std::cout << "  -f          Overwrite output file if it already exists."
            << std::endl;
  std::cout << std::endl;
  std::cout
      << "Split multi-property BTOR2 files into single property files. "
         "For each\nproperty a new file '<basename>p[0-9]+.btor is generated"
      << std::endl;
}

static void
die(const char *fmt, ...)
{
  va_list ap;
  va_start(ap, fmt);
  fprintf(stderr, "error: ");
  vfprintf(stderr, fmt, ap);
  va_end(ap);
  fprintf(stderr, "\n");
  exit(EXIT_FAILURE);
}

/*--------------------------------------------------------------------------*/

static bool
file_exists(std::string &filename)
{
  struct stat buf;
  return stat(filename.c_str(), &buf) != -1;
}

static void
split_file(std::string infilename, bool overwrite)
{
  size_t pos;
  std::ifstream infile(infilename);
  std::vector<std::string> lines;
  std::unordered_set<size_t> bad;

  std::string line;
  while (std::getline(infile, line))
  {
    pos = line.find("bad");
    if (pos != line.npos)
    {
      bad.insert(lines.size());
    }
    lines.push_back(line);
  }

  if (bad.size() <= 1)
  {
    std::cout << "Found only one property. Nothing to split" << std::endl;
    return;
  }

  pos = infilename.rfind(".");

  std::string prefix = infilename.substr(0, pos);
  std::string suffix = infilename.substr(pos, infilename.length());

  if (s_verbosity)
  {
    std::cout << "Found " << bad.size() << " properties in " << lines.size()
              << " lines" << std::endl;
  }

  uint32_t ndigits = 0;
  /* Note: We don't care that ndigits is 0 if bad.size() == 0. */
  for (size_t i = bad.size(); i > 0; i = i / 10, ++ndigits)
    ;

  size_t num_prop = 0;
  for (size_t lineno : bad)
  {
    std::stringstream ss;
    ss << prefix << "-p" << std::setfill('0') << std::setw(ndigits) << num_prop
       << suffix;

    std::string outfilename = ss.str();

    if (!overwrite && file_exists(outfilename))
    {
      die("Output file %s already exists. Delete or use -f to overwrite",
          outfilename.c_str());
    }

    std::ofstream outfile(outfilename);
    for (size_t i = 0; i < lines.size(); ++i)
    {
      if (bad.find(i) == bad.end() || lineno == i)
      {
        outfile << lines[i] << std::endl;
      }
    }
    outfile.close();
    ++num_prop;
    if (s_verbosity) std::cout << "Generated " << ss.str() << std::endl;
  }
}

int
main(int argc, char *argv[])
{
  bool overwrite = false;
  std::vector<std::string> infiles;

  for (int i = 1; i < argc; ++i)
  {
    if (!strcmp(argv[i], "-v"))
    {
      ++s_verbosity;
    }
    else if (!strcmp(argv[i], "-f"))
    {
      overwrite = true;
    }
    else if (!strcmp(argv[i], "-h") || !strcmp(argv[i], "--help"))
    {
      print_usage();
      return EXIT_SUCCESS;
    }
    else
    {
      infiles.push_back(std::string(argv[i]));
    }
  }

  if (infiles.empty())
  {
    die("No BTOR2 input file(s) specified.");
  }

  for (auto infile : infiles)
  {
    if (s_verbosity) std::cout << "Processing " << infile << std::endl;
    split_file(infile, overwrite);
  }

  return EXIT_SUCCESS;
}
