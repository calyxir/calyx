from pathlib import Path
from termcolor import colored, cprint
import shutil
import subprocess
from packaging import version


VERSIONS = {
    'dahlia': {
        'flag': '--version',
        'extract': lambda out: out,
        'version': 'dirty',
        'compare': 'status_is_not',
        'help': 'Dahlia binary built using uncommitted changes. Please commit changes to the Dahlia compiler and rebuild it.'
    },
    'verilog': {
        'flag': '--version',
        'extract': lambda out: out.split(' ')[1],
        'version': '4.100',
        'compare': '>='
    },
    'vcd': {
        'flag': '--version',
        'extract': lambda out: out.split(' ')[1],
        'version': '0.1.2',
        'compare': '>='
    },
    'vivado': {
        'flag': '-version',
        'extract': lambda out: out.split(' ')[1],
        'version': 'v2019.2',
        'compare': '=='
    },
    'vivado_hls': {
        'flag': '-version',
        'extract': lambda out: out.split(' ')[10],
        'version': 'v2019.2',
        'compare': '=='
    }
}


def version_compare(cmp_str, installed, required):
    if cmp_str == ">=":
        return version.parse(installed) >= version.parse(required)
    elif cmp_str == "==":
        return version.parse(installed) == version.parse(required)
    elif cmp_str == "<=":
        return version.parse(installed) <= version.parse(required)
    elif cmp_str == "status_is_not":
        return required not in installed


def check_version(name, exec_path):
    if name in VERSIONS:
        info = VERSIONS[name]
        proc = subprocess.run([exec_path, info['flag']], stdout=subprocess.PIPE)
        install = info['extract'](proc.stdout.decode('UTF-8')).strip()
        if version_compare(info['compare'], install, info['version']):
            cprint(" ✔", 'green', end=' ')
            print("Found version", end=' ')
            cprint(f"{install}", 'yellow', end=' ')
            print(f"({info['compare']} ", end='')
            cprint(f"{info['version']}", 'yellow', end='')
            print(")", end='')
            print(".")
            return True
        else:
            cprint(" ✖", 'red', end=' ')
            print("Found version", end=' ')
            cprint(f"{install},", 'yellow', end=' ')
            print(f"but need version {info['compare']} ", end='')
            cprint(f"{info['version']}", 'yellow', end='')
            print(".")
            cprint(f"   {info['help']}")
            return False
    else:
        return True


def check(args, cfg):
    cfg.launch_wizard()

    # check global
    futil_root = Path(cfg['global', 'futil_directory'])
    futil_str = colored(str(futil_root), 'yellow')
    cprint('global:', attrs=['bold'])
    if futil_root.exists():
        cprint(" ✔", 'green', end=' ')
        print(f"{futil_str} exists.")
    else:
        cprint(" ✖", 'red', end=' ')
        print(f"{futil_str} doesn't exist.")
    print()

    uninstalled = []
    wrong_version = []
    # check executables in stages
    for name, stage in cfg['stages'].items():
        if 'exec' in stage:
            cprint(f'stages.{name}.exec:', attrs=['bold'])
            exec_path = shutil.which(stage['exec'])
            exec_name = colored(stage['exec'], 'yellow')
            if exec_path is not None or stage['exec'].startswith('cargo run'):
                cprint(" ✔", 'green', end=' ')
                print(f"{exec_name} installed.")
                # check if path is absolute or relative
                if exec_path is not None and not Path(exec_path).is_absolute():
                    print(
                        f"   {exec_name} is a relative path and will not work from every directory.")
                # check version
                if not check_version(name, exec_path):
                    wrong_version.append(name)
            else:
                uninstalled.append(name)
                cprint(" ✖", 'red', end=' ')
                print(f"{exec_name} not installed.")
            print()
    if len(uninstalled) > 0:
        bad_stages = colored(', '.join(uninstalled), 'red')
        verb = 'were' if len(uninstalled) > 1 else 'was'
        print(f"{bad_stages} {verb} not installed correctly.")
        print("Configuration instructions: https://capra.cs.cornell.edu/calyx/tools/fud.html#configuration")

    if len(wrong_version) > 0:
        # add line break if we printed out uninstalled
        if len(uninstalled) > 0:
            print()
        bad_stages = colored(', '.join(wrong_version), 'red')
        verb = 'versions' if len(wrong_version) > 1 else 'version'
        print(f"Incorrect {verb} for {bad_stages}.")
        print("Stages may fail or not execute correctly.")

    # exit with -1 if something went wrong
    if len(uninstalled) > 0 or len(wrong_version) > 0:
        exit(-1)
