import os

for x in range(900, 1100):
  os.system(f"python3 calyx-py/calyx/queue_data_gen.py {x} 16 > calyx-py/test/correctness/queues/rr_queues/rr_queue.data")
  os.system(f"cat calyx-py/test/correctness/queues/rr_queues/rr_queue.data | python3 calyx-py/calyx/rrqueue_oracle.py {x} 16 --keepgoing  > calyx-py/test/correctness/queues/rr_queues/rr_queue.expect")

