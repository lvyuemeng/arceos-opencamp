ATOMIC ?= n
ITER ?= n
THREAD ?= n
ASYNC ?= n

PARSE_ARGS = 
FEATURES = 
POSTFIX =

ifeq ($(ATOMIC), y)
	PARSE_ARGS += --parser atomic-sum
	FEATURES += atomic-sum 
	POSTFIX += atomic
endif

ifeq ($(ITER), y)
	PARSE_ARGS += --parser iter-delay
	FEATURES += iter-delay 
	POSTFIX += iter
endif

ifeq ($(THREAD), y)
	FEATURES += thread-test 
endif

ifeq ($(ASYNC), y)
	FEATURES += async-test 
endif

NUM_ENABLED = $(words  $(filter y, $(ATOMIC) $(ITER)))

ifeq ($(NUM_ENABLED), 0)
  $(error Please set at least one test parser to run. Use ATOMIC=y or ITER=y.)
else ifneq ($(NUM_ENABLED), 1)
  $(error Please set only one test parser to run. Use ATOMIC=y or ITER=y.)
endif

RUN_CMD = $(MAKE) -C ../../../ A=$(shell pwd)/../ ARCH=riscv64 run LOG=info APP_FEATURES="$(FEATURES)"

BASE_NAME ?= log
SUMMARY ?= sum

RAW_LOGS = raw_$(BASE_NAME)_$(POSTFIX).txt
CLEANED_LOGS = $(BASE_NAME)_$(POSTFIX).txt
SUMMARY_CSV = $(BASE_NAME)_$(POSTFIX).csv

all: run purify analyze

run:
	$(RUN_CMD) > $(RAW_LOGS)

# purify the ansi chars
purify:
	sed -r 's/\x1B\[([0-9]{1,2}(;[0-9]{1,2})?)?[m|K]//g' $(RAW_LOGS) | \
	sed -r 's/^\[ [0-9\.]+ [0-9:]+ [^\]]+\] (.*)/\1/' > $(CLEANED_LOGS)
	rm -f $(RAW_LOGS)

analyze:
	python3 log_parse.py "$(CLEANED_LOGS)" $(PARSE_ARGS) -o "$(SUMMARY_CSV)"
	
clean:
	rm -f $(CLEANED_LOGS)

.PHONY: run purify analyze clean