<?php

declare(strict_types=1);
// Target: PHP 8.4

namespace TypePhp\MicroFw\Http\Message;

interface UriInterface
{
    public function getScheme(): string;
    public function getHost(): string;
    public function getPath(): string;
    public function getQuery(): string;
}
