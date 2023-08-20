import ssl
import subprocess
import sys

from aiosmtpd.controller import Controller

# An SMTP server that writes each received email to a file.
# Assists with end-to-end testing.
# Inspired by https://aiosmtpd.readthedocs.io/en/latest/migrating.html

class CustomHandler:
    def __init__(self, directory):
        self._directory = directory
        
    async def handle_DATA(self, server, session, envelope):
        peer = session.peer
        mail_from = envelope.mail_from
        rcpt_tos = envelope.rcpt_tos
        data = envelope.content
        print(self._directory, peer, mail_from, rcpt_tos, data)
        return '250 OK'

if __name__ == '__main__':
    handler = CustomHandler(sys.argv[1])
    # TLS details from https://stackoverflow.com/questions/45447491/how-do-i-properly-support-starttls-with-aiosmtpd
    subprocess.call('openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem ' +
                    '-days 365 -nodes -subj "/CN=localhost"', shell=True)
    context = ssl.create_default_context(ssl.Purpose.CLIENT_AUTH)
    context.load_cert_chain('cert.pem', 'key.pem')
    from aiosmtpd.smtp import SMTP
    class ControllerStarttls(Controller):
        def factory(self):
            return SMTP(self.handler, require_starttls=True, tls_context=context)
    controller = ControllerStarttls(handler, hostname='127.0.0.1', port=10025)
    #controller = Controller(handler, hostname='127.0.0.1', port=10025, ssl_context=context)
    # Run the event loop in a separate thread.
    controller.start()
    # Wait for the user to press Return.
    input('SMTP server running. Press Return to stop server and exit.')
    controller.stop()
