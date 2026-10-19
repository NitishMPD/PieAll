# Performance verification
Author: Nitish

No runtime performance numbers have been measured. Preserve the targets and definitions in PieAll-plan.md section 7. Record OS build, CPU/GPU/RAM, display DPI/refresh rate, build flags, signed artifact size, and cold/warm state for each measurement. Include every PieAll helper/worker in process-tree measurements. Measure first and warm invocation separately using ETW/presentation tracing; report idle five-minute CPU/private commit, preview peak, engine memory separately, and actual compressed/installed payload sizes. Unit-test elapsed time does not prove interaction latency.
