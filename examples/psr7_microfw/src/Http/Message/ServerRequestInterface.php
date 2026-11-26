<?php

declare(strict_types=1);
// Target: PHP 8.4

namespace TypePhp\MicroFw\Http\Message;

require_once __DIR__ . '/RequestInterface.php';

interface ServerRequestInterface extends RequestInterface
{
    /**
     * @return array<string, mixed>
     */
    public function getQueryParams(): array;
    public function getParsedBody(): mixed;
    public function getAttribute(string $name): mixed;
    public function withAttribute(string $name, mixed $value): ServerRequestInterface;
}
