#!/bin/bash
sudo docker build -t ratnadeepb/async_client .
sudo docker push ratnadeepb/async_client:latest

sudo docker run -v /tmp:/tmp -d ratnadeepb/async_client:latest