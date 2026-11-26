<?php

declare(strict_types=1);
// Target: PHP 8.4

namespace TypePhp\MicroFw\Validation;

require_once __DIR__ . '/ValidationResult.php';

class Validator
{
    /**
     * @return string|null
     */
    private function checkSingleRule(string $fieldName, mixed $val, string $rule, bool $isRequired): ?string
    {
        if ($rule === 'string') {
            if (!is_string($val)) {
                return $fieldName . ' must be a string';
            }
            if ($isRequired && trim($val) === '') {
                return $fieldName . ' cannot be empty';
            }
        } elseif ($rule === 'int') {
            if (!is_int($val)) {
                return $fieldName . ' must be an integer';
            }
        } elseif ($rule === 'bool') {
            if (!is_bool($val)) {
                return $fieldName . ' must be a boolean';
            }
        } elseif (str_starts_with($rule, 'min:')) {
            $minLen = intval(substr($rule, 4));
            if (is_string($val)) {
                if (strlen(trim($val)) < $minLen) {
                    return $fieldName . ' must be at least ' . (string) $minLen . ' characters';
                }
            }
        } elseif (str_starts_with($rule, 'max:')) {
            $maxLen = intval(substr($rule, 4));
            if (is_string($val)) {
                if (strlen(trim($val)) > $maxLen) {
                    return $fieldName . ' must be at most ' . (string) $maxLen . ' characters';
                }
            }
        }
        return null;
    }

    /**
     * @return string|null
     */
    private function validateField(string $fieldName, mixed $val, string $ruleString): ?string
    {
        /** @var list<string> $parts */
        $parts = explode('|', $ruleString);
        $isRequired = false;
        foreach ($parts as $p) {
            if ($p === 'required') {
                $isRequired = true;
            }
        }
        foreach ($parts as $rule) {
            /** @var string|null $err */
            $err = $this->checkSingleRule($fieldName, $val, $rule, $isRequired);
            if ($err !== null) {
                return $err;
            }
        }
        return null;
    }

    /**
     * @param array<string, string> $rules
     */
    public function validate(mixed $input, array $rules): ValidationResult
    {
        /** @var array<string, string> $errors */
        $errors = [];
        /** @var array<string, mixed> $clean */
        $clean = [];
        if (!is_array($input)) {
            $errors['_body'] = 'Request body must be a JSON object';
            return new ValidationResult(false, $errors, $clean);
        }
        foreach ($rules as $field => $ruleString) {
            $fieldName = (string) $field;
            $hasField = array_key_exists($fieldName, $input);
            $val = null;
            if ($hasField) {
                $val = $input[$fieldName];
            }
            /** @var list<string> $parts */
            $parts = explode('|', $ruleString);
            $isRequired = false;
            foreach ($parts as $p) {
                if ($p === 'required') {
                    $isRequired = true;
                }
            }
            if (!$hasField || $val === null) {
                if ($isRequired) {
                    $errors[$fieldName] = $fieldName . ' is required';
                }
            } else {
                /** @var string|null $err */
                $err = $this->validateField($fieldName, $val, $ruleString);
                if ($err !== null) {
                    $errors[$fieldName] = $err;
                } elseif (is_string($val)) {
                    $clean[$fieldName] = trim($val);
                } else {
                    $clean[$fieldName] = $val;
                }
            }
        }
        $ok = true;
        foreach ($errors as $k => $v) {
            $ok = false;
        }
        return new ValidationResult($ok, $errors, $clean);
    }
}
