;; Test runner for the incremental compiler

(define file-asm "stst.s")
(define file-bin "stst")
(define file-out "stst.out")

(define (build)
  (unless (zero? (system "make --quiet"))
    (error 'make "Could not build target")))

(define (execute)
  (let ([command (format "./~a > ~a" file-bin file-out)])
    (unless (zero? (system command))
      (error 'make "Produced program exited abnormally"))))

;; Compile port is a parameter that returns a port to write the generated asm
;; to. This can be a file or stdout.
(define compile-port
  (make-parameter
   (current-output-port)
   (lambda (p)
     (unless (output-port? p)
       (error 'compile-port "Not an output port ~s" p))
     p)))

;; Run the compiler but send the output to a file instead
(define (compile-program expr)
  (let ([p (open-output-file file-asm 'replace)])
    (parameterize ([compile-port p])
      (emit-program expr))
    (close-output-port p)))

(define (emit . args)
  (let ([out (compile-port)])
    (apply fprintf out args)
    (newline out)))

;; This seems like a very creepy and odd way to read everything from the output
;; file character by character and print it out. Unless I'm missing something
;; else here ¯\_(ツ)_/¯
(define (get-string)
  (with-output-to-string
    (lambda ()
      (with-input-from-file file-out
        (lambda ()
          (let f ()
            (let ([c (read-char)])
              (cond
               [(eof-object? c) (void)]
               [else (display c) (f)]))))))))

;; Compile, build, execute and assert output with expectation
(define (test-with-string-output test-id expr expected-output)
  (compile-program expr)
  (build)
  (execute)
  (unless (string=? expected-output (get-string))
    (error 'test (format "Output mismatch for test ~s, expected ~s, got ~s"
                         test-id expected-output (get-string)))))

;; Collect all tests in a global variable
(define all-tests '())

(define-syntax add-tests-with-string-output
  (syntax-rules (=>)
    [(_ test-name [expr => output-string] ...)
     (set! all-tests
           (cons
            '(test-name [expr string output-string] ...)
            all-tests))]))

(define (test-one test-id test)
  (let ([expr (car test)]
        [type (cadr test)]
        [expected-output (caddr test)])
    (printf "Test ~s: ~s ..." test-id expr)
    (flush-output-port)
    (case type
      [(string) (test-with-string-output test-id expr expected-output)]
      [else (error 'test "invalid test type ~s" type)])
    (printf " ok\n")))

(define (test-all)
  (let f ([i 0] [ls (reverse all-tests)])
    (if (null? ls)
        (printf "Passed all ~s tests\n" i)
        (let ([x (car ls)] [ls (cdr ls)])
          (let* ([test-name (car x)]
                 [tests (cdr x)]
                 [n (length tests)])
            (printf "Performing ~a tests ...\n" test-name)
            (let g ([i i] [tests tests])
              (cond
                [(null? tests) (f i ls)]
                [else
                 (test-one i (car tests))
                 (g (add1 i) (cdr tests))])))))))
