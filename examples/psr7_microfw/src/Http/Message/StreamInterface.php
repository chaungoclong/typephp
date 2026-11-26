<?php

declare(strict_types=1);
// Target: PHP 8.4

namespace TypePhp\MicroFw\Http\Message;

interface StreamInterface
{
    public function __toString(): string;
    public function getContents(): string;
    public function getSize(): int;
}
