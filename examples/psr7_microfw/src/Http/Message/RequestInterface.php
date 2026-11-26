<?php

declare(strict_types=1);
// Target: PHP 8.4

namespace TypePhp\MicroFw\Http\Message;

require_once __DIR__ . '/MessageInterface.php';
require_once __DIR__ . '/UriInterface.php';

interface RequestInterface extends MessageInterface
{
    public function getMethod(): string;
    public function getUri(): UriInterface;
}
