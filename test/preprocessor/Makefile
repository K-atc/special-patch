TARGET = main main.expected

all: $(TARGET)
	@

%:%.c
	$(CC) -E -dI -C $^ -o $@.E
	$(CC) $^ -o $@

%.expected:%.E.expected
	$(CC) -x c $^ -o $@

clean:
	rm -f $(TARGET) *.l