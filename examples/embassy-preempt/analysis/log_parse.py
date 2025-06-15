import argparse
import re
import csv
import os
import sys
from collections import defaultdict
from statistics import mean, stdev
from datetime import datetime

# follow the same methods
class IterDelay:
    LOG_PATTERNS = {
        "async_report_high": {
            "regex": r'ASYNC_TASK_REPORT_HIGH (\d+): volume (\d+), works (\d+), times (\d+)/s, iters (\d+), expected (\d+)/ns, actual (\d+)/ns, full (\d+)/ns',
            "metrics": ["actual_ns", "full_ns"]
        },
        "async_report_low": {
            "regex": r'ASYNC_TASK_REPORT_LOW (\d+): volume (\d+), works (\d+), times (\d+)/s, iters (\d+), expected (\d+)/ns, actual (\d+)/ns, full (\d+)/ns',
            "metrics": ["actual_ns", "full_ns"]
        },
        "native_report": {
            "regex": r'NATIVE_THREAD_REPORT (\d+): volume (\d+), works (\d+), times (\d+)/s, iters (\d+), expected (\d+)/ns, actual (\d+)/ns, full (\d+)/ns',
            "metrics": ["actual_ns", "full_ns"]
        }
    }
    
    REGEX = { name: re.compile(config["regex"]) for name, config in LOG_PATTERNS.items() }
    
    CSV_HEADERS = [
        "timestamp",
        "log_type",
        "volume",
        "works",
        "time",
        "metric_name", 
        "count",
        "mean_absolute_deviation",
        "std_deviation",
        "min_deviation",
        "max_deviation"
    ]

    @staticmethod
    def parse_line(line: str, store: defaultdict):
        """
        Parse a line of log output and store the relevant information.

        :param line: a line of log output
        :param store: a dictionary where the parsed data will be stored.
                      Key: log_type,
                      Val: dict:
                            Key: metric_name, 
                            Val: (shared_info, dev)
        :return: None
        """
        for log_type, config in IterDelay.LOG_PATTERNS.items():
            match = IterDelay.REGEX[log_type].search(line)
            if match:
                (
                    _id, volume, works, time, iters, 
                    expected, *metrics
                ) = map(int, match.groups())
                deviations = [abs(value - expected) for value in metrics]
                shared_info = (volume, works, time)
                for metric, dev in zip(config["metrics"], deviations):
                    store[log_type][metric].append((shared_info,dev))
                return
        return
    
    @staticmethod
    def csv_row(parsed_datas: list[dict]) -> list[list]:
        timestamp = datetime.now().isoformat()

        data_rows = []
        for log_type, metrics in parsed_datas.items():
            for metric, entries in metrics.items():
                if entries:  # Only write if we have data
                    (volume, works, time), *_ = [info for info, _ in entries]
                    devs = [dev for _, dev in entries]

                    std_dev = stdev(devs) if len(devs) > 1 else 0
                    row = [
                        timestamp,
                        log_type,
                        volume,
                        works,
                        time,
                        metric,
                        len(devs),
                        round(mean(devs),2),
                        round(std_dev,2),
                        min(devs),
                        max(devs)
                    ]
                    
                    data_rows.append(row)
        return data_rows

    @staticmethod
    def display_summary(csv_rows: list[list]):
        if not csv_rows:
            print("No data to display.")
            return

        for row in csv_rows:
            print(f"--- {row[1]} ---")
            print(f"  Metric: {row[5]}")
            print(f"    Volume: {row[2]}")
            print(f"    Works: {row[3]}")
            print(f"    Time: {row[4]} /s")
            print(f"    Count: {row[6]}")
            print(f"    Mean Absolute Deviation: {row[7]:.2f} ns")
            print(f"    Std Dev: {row[8]:.2f} ns" if row[8] > 0 else "    Std Dev: N/A")
            print(f"    Min: {row[9]} ns")
            print(f"    Max: {row[10]} ns")
            print()

class AtomicSum:
    LOG_PATTERNS = {
        "async_report_high": {
            "regex": r'ASYNC_TASK_REPORT_HIGH: volume: (\d+), time: (\d+)/s, works: (\d+), sum: (\d+), sum/s: (\d+)',
            "id_key": "task_id",
            "metrics": ["volume", "sum", "sum_per_secs"]
        },
        "async_report_low": {
            "regex": r'ASYNC_TASK_REPORT_LOW: volume: (\d+), time: (\d+)/s, works: (\d+), sum: (\d+), sum/s: (\d+)',
            "id_key": "task_id",
            "metrics": ["volume", "sum", "sum_per_secs"]
        },
        "native_report": {
            "regex": r'NATIVE_THREAD_REPORT: volume: (\d+), time: (\d+)/s, works: (\d+), sum: (\d+), sum/s: (\d+)',
            "id_key": "thread_id",
            "metrics": ["volume", "sum", "sum_per_secs"]
        }
    }
    
    REGEX = { name: re.compile(config["regex"]) for name, config in LOG_PATTERNS.items() }

    CSV_HEADERS = [
        "timestamp",
        "log_type",
        "volume",
        "time",
        "sum",
        "sum_per_secs",
    ]
    
    @staticmethod
    def parse_line(line, store: defaultdict) -> dict | None:
        """
        Parse a line of log output and store the relevant information.

        :param line: a line of log output
        :param store: a dictionary where the parsed data will be stored.
                      Key: log_type,
                      Val: metrics: volume, time, works, sum, sum_per_secs
        :return: None
        """
        for log_type, config in AtomicSum.LOG_PATTERNS.items():
            match = AtomicSum.REGEX[log_type].search(line)
            if match:
                metrics = map(int, match.groups())
                for value in metrics:
                    store[log_type]["sum_per_secs"].append(value)
                return
        return

    @staticmethod
    def csv_row(parsed_datas: list[dict]) -> list[list]:
        timestamp = datetime.now().isoformat()

        data_rows = []
        for log_type, metrics in parsed_datas.items():
            for metric, entries in metrics.items():
                if entries:
                    row = [
                        timestamp,
                        log_type,
                        *entries
                    ]
                data_rows.append(row)
        return data_rows

    @staticmethod
    def display_summary(csv_rows: list[list]):
        if not csv_rows:
            print("No data to display.")
            return

        for row in csv_rows:
            print(f"--- {row[1]} ---")
            print(f"  Volume: {row[2]}")
            print(f"  Time: {row[3]} ns/s")
            print(f"  Work: {row[4]} ns/s")
            print(f"  Sum: {row[5]}")
            print(f"  Sum/s: {row[6]}")
            print()
    
def parse_log_file(path: str, parser) -> defaultdict:
    parsed_datas = defaultdict(lambda: defaultdict(list))

    try:
        with open(path, "r") as f:
            for line in f:
                parser.parse_line(line, parsed_datas)
    except FileNotFoundError:
        print(f"Error: Log file '{path}' not found.", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"An error occurred while parsing the log file: {e}", file=sys.stderr)
        sys.exit(1)

    return parsed_datas

def write_to_csv(rows, log_path, csv_path, parser):
    log_filename = os.path.basename(log_path)
    
    file_exists = os.path.exists(csv_path)
    
    try:
        with open(csv_path, 'a', newline='') as csvfile:
            writer = csv.writer(csvfile)
            
            # Write headers if file is new
            if not file_exists:
                writer.writerow(parser.CSV_HEADERS)
            
            writer.writerows(rows)
                        
    except Exception as e:
        print(f"Error writing to CSV file '{csv_file_path}': {e}", file=sys.stderr)
        sys.exit(1)
        
PARSERS = {
    "iter-delay": IterDelay,
    "atomic-sum": AtomicSum,
}

def main():
    parser = argparse.ArgumentParser(description="Parse log file and write to CSV.")
    parser.add_argument("log_file", help="Path to the log file.")
    parser.add_argument("--parser", choices=PARSERS.keys(), required=True, help="Parser to parse the log file.")
    parser.add_argument("-o", "--output", default="output.csv", help="CSV output file path (default: output.csv).")
    args = parser.parse_args()
    
    parser_cls = PARSERS[args.parser]

    try:
        log_path = args.log_file
        csv_path = args.output
        data_set = parse_log_file(log_path, parser_cls)
        if not any(data_set.values()):
            print("No matching log entries found in the file.", file=sys.stderr)
            sys.exit(1)

        rows = parser_cls.csv_row(data_set)
        parser_cls.display_summary(rows)
        write_to_csv(rows, log_path, csv_path, parser_cls)
    except FileNotFoundError:
        print(f"Error: File '{sys.argv[1]}' not found.", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    main()