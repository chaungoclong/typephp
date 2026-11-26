<?php

declare(strict_types=1);
// Target: PHP 8.4

namespace TypePhp\MicroFw\Validation;

class ValidationResult
{
    private bool $valid;
    /** @var array<string, string> */
    private array $errors;
    /** @var array<string, mixed> */
    private array $data;

    /**
     * @param array<string, string> $errors
     * @param array<string, mixed> $data
     */
    public function __construct(bool $valid, array $errors, array $data)
    {
        $this->valid = $valid;
        $this->errors = $errors;
        $this->data = $data;
    }

    public function isValid(): bool
    {
        return $this->valid;
    }

    /**
     * @return array<string, string>
     */
    public function getErrors(): array
    {
        return $this->errors;
    }

    /**
     * @return array<string, mixed>
     */
    public function getData(): array
    {
        return $this->data;
    }
}
